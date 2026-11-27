export interface JwtParts {
  header: Record<string, unknown>;
  payload: Record<string, unknown>;
  signature: string;
  signingInput: string;
}

export interface JwtVerifyResult {
  valid: boolean;
  algorithm: string;
  message: string;
}

function bufferSource(bytes: Uint8Array): BufferSource {
  return bytes as unknown as BufferSource;
}

function base64UrlToBytes(value: string): Uint8Array {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function base64UrlToText(value: string): string {
  return new TextDecoder().decode(base64UrlToBytes(value));
}

export function decodeJwt(token: string): JwtParts {
  const parts = token.trim().split(".");
  if (parts.length !== 3) throw new Error("jwtParts");
  const header = JSON.parse(base64UrlToText(parts[0])) as Record<string, unknown>;
  const payload = JSON.parse(base64UrlToText(parts[1])) as Record<string, unknown>;
  return {
    header,
    payload,
    signature: parts[2],
    signingInput: `${parts[0]}.${parts[1]}`,
  };
}

function hmacAlgorithm(algorithm: string): string {
  if (algorithm === "HS256") return "SHA-256";
  if (algorithm === "HS384") return "SHA-384";
  if (algorithm === "HS512") return "SHA-512";
  throw new Error("jwtUnsupportedAlgorithm");
}

export async function verifyJwt(token: string, key: string): Promise<JwtVerifyResult> {
  const parts = decodeJwt(token);
  const algorithm = String(parts.header.alg ?? "");
  const data = bufferSource(new TextEncoder().encode(parts.signingInput));
  const signature = bufferSource(base64UrlToBytes(parts.signature));

  if (algorithm.startsWith("HS")) {
    const cryptoKey = await crypto.subtle.importKey(
      "raw",
      bufferSource(new TextEncoder().encode(key)),
      { name: "HMAC", hash: hmacAlgorithm(algorithm) },
      false,
      ["verify"],
    );
    const valid = await crypto.subtle.verify("HMAC", cryptoKey, signature, data);
    return { valid, algorithm, message: valid ? "jwtSignatureValid" : "jwtSignatureInvalid" };
  }

  if (algorithm.startsWith("RS") || algorithm.startsWith("PS") || algorithm.startsWith("ES")) {
    let jwk: JsonWebKey;
    try {
      jwk = JSON.parse(key) as JsonWebKey;
    } catch {
      throw new Error("jwtPublicKeyRequired");
    }
    const hash = algorithm.endsWith("384")
      ? "SHA-384"
      : algorithm.endsWith("512")
        ? "SHA-512"
        : "SHA-256";
    const importAlgorithm = algorithm.startsWith("ES")
      ? { name: "ECDSA", namedCurve: String(jwk.crv ?? "P-256") }
      : algorithm.startsWith("PS")
        ? { name: "RSA-PSS", hash }
        : { name: "RSASSA-PKCS1-v1_5", hash };
    const verifyAlgorithm = algorithm.startsWith("ES")
      ? { name: "ECDSA", hash }
      : algorithm.startsWith("PS")
        ? { name: "RSA-PSS", saltLength: hash === "SHA-512" ? 64 : hash === "SHA-384" ? 48 : 32 }
        : importAlgorithm;
    const cryptoKey = await crypto.subtle.importKey("jwk", jwk, importAlgorithm, false, ["verify"]);
    const valid = await crypto.subtle.verify(verifyAlgorithm, cryptoKey, signature, data);
    return { valid, algorithm, message: valid ? "jwtSignatureValid" : "jwtSignatureInvalid" };
  }

  throw new Error("jwtUnsupportedAlgorithm");
}

export interface JwtClaimCheck {
  expired: boolean;
  notBefore: boolean;
  expiresAt?: Date;
  issuedAt?: Date;
}

export function checkJwtClaims(payload: Record<string, unknown>, now = new Date()): JwtClaimCheck {
  const nowSeconds = Math.floor(now.getTime() / 1000);
  const exp = typeof payload.exp === "number" ? payload.exp : undefined;
  const nbf = typeof payload.nbf === "number" ? payload.nbf : undefined;
  const iat = typeof payload.iat === "number" ? payload.iat : undefined;
  return {
    expired: exp !== undefined && exp <= nowSeconds,
    notBefore: nbf !== undefined && nbf > nowSeconds,
    expiresAt: exp !== undefined ? new Date(exp * 1000) : undefined,
    issuedAt: iat !== undefined ? new Date(iat * 1000) : undefined,
  };
}
