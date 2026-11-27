import { describe, expect, it } from "vitest";
import { checkJwtClaims, decodeJwt, verifyJwt } from "./jwt";

const TOKEN =
  "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9." +
  "eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ." +
  "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

describe("jwt helpers", () => {
  it("decodes header and payload", () => {
    const parts = decodeJwt(TOKEN);
    expect(parts.header.alg).toBe("HS256");
    expect(parts.payload.name).toBe("John Doe");
  });

  it("verifies an HS256 signature", async () => {
    const result = await verifyJwt(TOKEN, "your-256-bit-secret");
    expect(result.valid).toBe(true);
  });

  it("rejects a wrong secret", async () => {
    const result = await verifyJwt(TOKEN, "wrong-secret");
    expect(result.valid).toBe(false);
  });

  it("rejects malformed tokens", () => {
    expect(() => decodeJwt("abc.def")).toThrow("jwtParts");
  });

  it("detects expired claims", () => {
    const claims = checkJwtClaims({ exp: 100, nbf: 300, iat: 50 }, new Date(200 * 1000));
    expect(claims.expired).toBe(true);
    expect(claims.notBefore).toBe(true);
  });
});
