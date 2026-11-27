use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ::time::OffsetDateTime;
use ::time::format_description::well_known::Rfc3339;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as TlsError, SignatureScheme};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::*;

use super::model::{SslCertDto, SslCertInput, SslCertResultDto, ToolErrorDto, ToolResult};

#[derive(Debug)]
struct AcceptAnyCertificate {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for AcceptAnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        // The purpose of this tool is inspection, including expired or
        // self-signed certificates. Trust decisions are shown to the user.
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub async fn inspect(input: SslCertInput) -> ToolResult<SslCertResultDto> {
    let host = input.host.trim().to_owned();
    if host.is_empty() {
        return Err(ToolErrorDto::new("invalidHost", "Host is required"));
    }
    let server_name_text = input
        .server_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&host)
        .to_owned();
    let started = Instant::now();

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|error| ToolErrorDto::new("tlsConfig", error.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCertificate { provider }))
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let server_name = ServerName::try_from(server_name_text.clone())
        .map_err(|error| ToolErrorDto::new("invalidServerName", error.to_string()))?;

    let timeout = Duration::from_millis(input.timeout_ms.clamp(2_000, 30_000));
    let tcp = tokio::time::timeout(timeout, TcpStream::connect((host.as_str(), input.port)))
        .await
        .map_err(|_| ToolErrorDto::new("tlsTimeout", "TCP connection timed out"))?
        .map_err(|error| ToolErrorDto::new("tlsConnect", error.to_string()))?;
    let stream = tokio::time::timeout(timeout, connector.connect(server_name, tcp))
        .await
        .map_err(|_| ToolErrorDto::new("tlsTimeout", "TLS handshake timed out"))?
        .map_err(|error| ToolErrorDto::new("tlsHandshake", error.to_string()))?;

    let connection = stream.get_ref().1;
    let tls_version = connection
        .protocol_version()
        .map(|value| format!("{value:?}"));
    let cipher_suite = connection
        .negotiated_cipher_suite()
        .map(|value| format!("{:?}", value.suite()));
    let certificates: Vec<CertificateDer<'static>> = connection
        .peer_certificates()
        .map(|certs| certs.to_vec())
        .unwrap_or_default();

    if certificates.is_empty() {
        return Err(ToolErrorDto::new(
            "tlsNoCertificate",
            "The peer did not present a certificate",
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let mut chain = Vec::new();
    for certificate in &certificates {
        match parse_certificate(certificate.as_ref(), now) {
            Ok(item) => chain.push(item),
            Err(error) => {
                chain.push(SslCertDto {
                    subject: "—".to_owned(),
                    issuer: "—".to_owned(),
                    serial: "—".to_owned(),
                    not_before: "—".to_owned(),
                    not_after: "—".to_owned(),
                    days_remaining: 0,
                    expired: false,
                    san: Vec::new(),
                    signature_algorithm: "—".to_owned(),
                    public_key_algorithm: "—".to_owned(),
                    is_ca: false,
                    self_signed: false,
                });
                tracing::warn!(%error, "failed to parse one certificate in chain");
            }
        }
    }

    Ok(SslCertResultDto {
        host,
        port: input.port,
        server_name: server_name_text,
        tls_version,
        cipher_suite,
        chain,
        elapsed_ms: round_ms(started.elapsed().as_secs_f64() * 1000.0),
    })
}

fn parse_certificate(der: &[u8], now: i64) -> Result<SslCertDto, String> {
    let (_, certificate) = X509Certificate::from_der(der).map_err(|error| error.to_string())?;
    let not_before = certificate.validity().not_before.timestamp();
    let not_after = certificate.validity().not_after.timestamp();
    let sanction = certificate
        .subject_alternative_name()
        .ok()
        .flatten()
        .map(|extension| {
            extension
                .value
                .general_names
                .iter()
                .filter_map(|name| match name {
                    GeneralName::DNSName(value) => Some((*value).to_owned()),
                    GeneralName::IPAddress(bytes) if bytes.len() == 4 => Some(format!(
                        "{}.{}.{}.{}",
                        bytes[0], bytes[1], bytes[2], bytes[3]
                    )),
                    GeneralName::IPAddress(bytes) if bytes.len() == 16 => {
                        let mut octets = [0u8; 16];
                        octets.copy_from_slice(bytes);
                        Some(std::net::Ipv6Addr::from(octets).to_string())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(SslCertDto {
        subject: certificate.subject().to_string(),
        issuer: certificate.issuer().to_string(),
        serial: certificate.raw_serial_as_string(),
        not_before: format_timestamp(not_before),
        not_after: format_timestamp(not_after),
        days_remaining: (not_after - now).div_euclid(86_400),
        expired: not_after < now,
        san: sanction,
        signature_algorithm: certificate.signature_algorithm.algorithm.to_id_string(),
        public_key_algorithm: certificate.public_key().algorithm.algorithm.to_id_string(),
        is_ca: certificate.is_ca(),
        self_signed: certificate.subject() == certificate.issuer(),
    })
}

fn format_timestamp(seconds: i64) -> String {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(|| seconds.to_string())
}

fn round_ms(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_timestamps() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn rounds_milliseconds() {
        assert_eq!(round_ms(12.345), 12.3);
    }
}
