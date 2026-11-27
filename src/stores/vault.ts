import { defineStore } from "pinia";
import { computed, ref } from "vue";

const STORAGE_KEY = "mt.vault.v1";
const VERSION = 1;
const ITERATIONS = 200_000;

export interface VaultEntry {
  id: string;
  name: string;
  username: string;
  password: string;
  url: string;
  note: string;
  createdAt: number;
  updatedAt: number;
}

interface StoredEntry {
  id: string;
  name: string;
  username: string;
  url: string;
  createdAt: number;
  updatedAt: number;
  iv: string;
  cipher: string;
}

interface VaultFile {
  version: typeof VERSION;
  salt: string;
  checkIv: string;
  checkCipher: string;
  entries: StoredEntry[];
}

interface EncryptedPayload {
  password: string;
  note: string;
}

function storage(): Storage | null {
  return typeof localStorage === "undefined" ? null : localStorage;
}

function bufferSource(bytes: Uint8Array): BufferSource {
  return bytes as unknown as BufferSource;
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function base64ToBytes(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

async function deriveKey(
  password: string,
  salt: Uint8Array,
  usage: "encrypt" | "verify",
): Promise<CryptoKey> {
  const material = await crypto.subtle.importKey(
    "raw",
    bufferSource(new TextEncoder().encode(password)),
    "PBKDF2",
    false,
    ["deriveKey"],
  );
  const additional = new TextEncoder().encode(usage === "verify" ? "vault-check" : "vault-data");
  const combined = new Uint8Array(salt.length + additional.length);
  combined.set(salt, 0);
  combined.set(additional, salt.length);
  return crypto.subtle.deriveKey(
    { name: "PBKDF2", salt: bufferSource(combined), iterations: ITERATIONS, hash: "SHA-256" },
    material,
    { name: "AES-GCM", length: 256 },
    false,
    usage === "verify" ? ["encrypt", "decrypt"] : ["encrypt", "decrypt"],
  );
}

async function encryptText(key: CryptoKey, text: string): Promise<{ iv: string; cipher: string }> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const cipher = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: bufferSource(iv) },
    key,
    bufferSource(new TextEncoder().encode(text)),
  );
  return { iv: bytesToBase64(iv), cipher: bytesToBase64(new Uint8Array(cipher)) };
}

async function decryptText(key: CryptoKey, iv: string, cipher: string): Promise<string> {
  const plain = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: bufferSource(base64ToBytes(iv)) },
    key,
    bufferSource(base64ToBytes(cipher)),
  );
  return new TextDecoder().decode(plain);
}

function readFile(): VaultFile | null {
  const store = storage();
  if (!store) return null;
  try {
    const raw = store.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as VaultFile;
    if (parsed.version !== VERSION || !Array.isArray(parsed.entries)) return null;
    return parsed;
  } catch {
    return null;
  }
}

function writeFile(file: VaultFile): void {
  storage()?.setItem(STORAGE_KEY, JSON.stringify(file));
}

export const useVaultStore = defineStore("vault", () => {
  const file = ref<VaultFile | null>(readFile());
  const dataKey = ref<CryptoKey | null>(null);
  const entries = ref<VaultEntry[]>([]);
  const configured = computed(() => file.value !== null);
  const unlocked = computed(() => dataKey.value !== null);

  async function decryptEntries(currentFile: VaultFile, key: CryptoKey): Promise<VaultEntry[]> {
    const result: VaultEntry[] = [];
    for (const stored of currentFile.entries) {
      try {
        const text = await decryptText(key, stored.iv, stored.cipher);
        const payload = JSON.parse(text) as EncryptedPayload;
        result.push({
          id: stored.id,
          name: stored.name,
          username: stored.username,
          url: stored.url,
          createdAt: stored.createdAt,
          updatedAt: stored.updatedAt,
          password: payload.password,
          note: payload.note ?? "",
        });
      } catch {
        // A single corrupted entry should not make the whole vault unusable.
      }
    }
    return result;
  }

  async function setMasterPassword(nextPassword: string, currentPassword?: string): Promise<void> {
    if (nextPassword.length < 6) throw new Error("vaultPasswordTooShort");
    const salt = crypto.getRandomValues(new Uint8Array(16));
    const verifyKey = await deriveKey(nextPassword, salt, "verify");
    const dataKeyNext = await deriveKey(nextPassword, salt, "encrypt");
    const check = await encryptText(verifyKey, "modbus-tool-vault");

    let nextEntries: StoredEntry[] = [];
    if (file.value) {
      if (!currentPassword) throw new Error("vaultPasswordRequired");
      const existing = await deriveKey(currentPassword, base64ToBytes(file.value.salt), "encrypt");
      const existingCheck = await deriveKey(
        currentPassword,
        base64ToBytes(file.value.salt),
        "verify",
      );
      try {
        await decryptText(existingCheck, file.value.checkIv, file.value.checkCipher);
      } catch {
        throw new Error("vaultWrongPassword");
      }
      const decrypted = await decryptEntries(file.value, existing);
      nextEntries = await Promise.all(
        decrypted.map(async (entry) => {
          const payload = JSON.stringify({ password: entry.password, note: entry.note });
          const encrypted = await encryptText(dataKeyNext, payload);
          return {
            id: entry.id,
            name: entry.name,
            username: entry.username,
            url: entry.url,
            createdAt: entry.createdAt,
            updatedAt: entry.updatedAt,
            iv: encrypted.iv,
            cipher: encrypted.cipher,
          };
        }),
      );
    }

    const nextFile: VaultFile = {
      version: VERSION,
      salt: bytesToBase64(salt),
      checkIv: check.iv,
      checkCipher: check.cipher,
      entries: nextEntries,
    };
    writeFile(nextFile);
    file.value = nextFile;
    dataKey.value = dataKeyNext;
    entries.value = await decryptEntries(nextFile, dataKeyNext);
  }

  async function unlock(password: string): Promise<boolean> {
    if (!file.value) return false;
    const verifyKey = await deriveKey(password, base64ToBytes(file.value.salt), "verify");
    try {
      await decryptText(verifyKey, file.value.checkIv, file.value.checkCipher);
    } catch {
      return false;
    }
    const key = await deriveKey(password, base64ToBytes(file.value.salt), "encrypt");
    dataKey.value = key;
    entries.value = await decryptEntries(file.value, key);
    return true;
  }

  function lock(): void {
    dataKey.value = null;
    entries.value = [];
  }

  function requireUnlocked(): CryptoKey {
    if (!dataKey.value) throw new Error("vaultLocked");
    return dataKey.value;
  }

  async function addEntry(
    input: Omit<VaultEntry, "id" | "createdAt" | "updatedAt">,
  ): Promise<VaultEntry> {
    const key = requireUnlocked();
    if (!file.value) throw new Error("vaultNotConfigured");
    const now = Date.now();
    const id = crypto.randomUUID();
    const encrypted = await encryptText(
      key,
      JSON.stringify({ password: input.password, note: input.note }),
    );
    const stored: StoredEntry = {
      id,
      name: input.name,
      username: input.username,
      url: input.url,
      createdAt: now,
      updatedAt: now,
      iv: encrypted.iv,
      cipher: encrypted.cipher,
    };
    const nextFile: VaultFile = { ...file.value, entries: [stored, ...file.value.entries] };
    writeFile(nextFile);
    file.value = nextFile;
    const entry: VaultEntry = { ...input, id, createdAt: now, updatedAt: now };
    entries.value = [entry, ...entries.value];
    return entry;
  }

  async function updateEntry(id: string, patch: Partial<Omit<VaultEntry, "id">>): Promise<void> {
    const key = requireUnlocked();
    if (!file.value) throw new Error("vaultNotConfigured");
    const current = entries.value.find((entry) => entry.id === id);
    if (!current) throw new Error("vaultEntryMissing");
    const next: VaultEntry = { ...current, ...patch, updatedAt: Date.now() };
    const encrypted = await encryptText(
      key,
      JSON.stringify({ password: next.password, note: next.note }),
    );
    const nextFile: VaultFile = {
      ...file.value,
      entries: file.value.entries.map((stored) =>
        stored.id === id
          ? {
              ...stored,
              name: next.name,
              username: next.username,
              url: next.url,
              updatedAt: next.updatedAt,
              iv: encrypted.iv,
              cipher: encrypted.cipher,
            }
          : stored,
      ),
    };
    writeFile(nextFile);
    file.value = nextFile;
    entries.value = entries.value.map((entry) => (entry.id === id ? next : entry));
  }

  function removeEntry(id: string): void {
    if (!file.value) return;
    const nextFile = {
      ...file.value,
      entries: file.value.entries.filter((entry) => entry.id !== id),
    };
    writeFile(nextFile);
    file.value = nextFile;
    entries.value = entries.value.filter((entry) => entry.id !== id);
  }

  return {
    configured,
    unlocked,
    entries,
    setMasterPassword,
    unlock,
    lock,
    addEntry,
    updateEntry,
    removeEntry,
  };
});
