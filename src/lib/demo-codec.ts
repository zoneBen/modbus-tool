import type { ByteOrder, DecodedFrame, FrameField, Framing } from "@/types/models";

export function crc16Modbus(bytes: readonly number[]): number {
  let crc = 0xffff;
  for (const byte of bytes) {
    crc ^= byte & 0xff;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = crc & 1 ? (crc >> 1) ^ 0xa001 : crc >> 1;
    }
  }
  return crc & 0xffff;
}

export function crcBytes(bytes: readonly number[]): [number, number] {
  const crc = crc16Modbus(bytes);
  return [crc & 0xff, crc >> 8];
}

export function lrcByte(bytes: readonly number[]): number {
  let sum = 0;
  for (const byte of bytes) sum = (sum + byte) & 0xff;
  return ((~sum + 1) & 0xff) >>> 0;
}

export function mbapFrame(transactionId: number, unitId: number, pdu: readonly number[]): number[] {
  const length = pdu.length + 1;
  return [
    (transactionId >> 8) & 0xff,
    transactionId & 0xff,
    0,
    0,
    (length >> 8) & 0xff,
    length & 0xff,
    unitId & 0xff,
    ...pdu,
  ];
}

export function rtuFrame(unitId: number, pdu: readonly number[]): number[] {
  const body = [unitId & 0xff, ...pdu];
  return [...body, ...crcBytes(body)];
}

export function asciiFrame(unitId: number, pdu: readonly number[]): number[] {
  const body = [unitId & 0xff, ...pdu];
  const frame = [...body, lrcByte(body)];
  const text = `:${frame.map((byte) => byte.toString(16).toUpperCase().padStart(2, "0")).join("")}\r\n`;
  return Array.from(new TextEncoder().encode(text));
}

export function readRequestPdu(address: number, quantity: number, functionCode = 3): number[] {
  return [
    functionCode & 0xff,
    (address >> 8) & 0xff,
    address & 0xff,
    (quantity >> 8) & 0xff,
    quantity & 0xff,
  ];
}

export function readResponsePdu(functionCode: number, dataBytes: readonly number[]): number[] {
  return [functionCode & 0xff, dataBytes.length & 0xff, ...dataBytes];
}

export function writeSingleRequestPdu(address: number, value: number): number[] {
  return [0x06, (address >> 8) & 0xff, address & 0xff, (value >> 8) & 0xff, value & 0xff];
}

export function wordsToDataBytes(words: readonly number[]): number[] {
  const bytes: number[] = [];
  for (const word of words) {
    bytes.push((word >> 8) & 0xff, word & 0xff);
  }
  return bytes;
}

function swapBytes(word: number): number {
  return ((word & 0xff) << 8) | ((word >> 8) & 0xff);
}

export function applyByteOrder(words: readonly number[], order: ByteOrder): number[] {
  const copy = [...words];
  switch (order) {
    case "abcd":
      return copy;
    case "cdab":
      return copy.reverse();
    case "badc":
      return copy.map(swapBytes);
    case "dcba":
      return copy.reverse().map(swapBytes);
    default:
      return copy;
  }
}

export function uint16Words(value: number, order: ByteOrder): number[] {
  return applyByteOrder([value & 0xffff], order);
}

export function floatToWords(value: number, order: ByteOrder): number[] {
  const buffer = new ArrayBuffer(4);
  const view = new DataView(buffer);
  view.setFloat32(0, value, false);
  const words = [
    (view.getUint8(0) << 8) | view.getUint8(1),
    (view.getUint8(2) << 8) | view.getUint8(3),
  ];
  return applyByteOrder(words, order);
}

export function decodeFloatFromWords(words: readonly number[]): number {
  if (words.length < 2) return Number.NaN;
  const bytes = wordsToDataBytes(words.slice(0, 2));
  const view = new DataView(new Uint8Array(bytes).buffer);
  return view.getFloat32(0, false);
}

function hex(value: number, digits = 2): string {
  return value.toString(16).toUpperCase().padStart(digits, "0");
}

function decodePayload(data: number[]): { label: string; value: string } {
  if (data.length === 0) return { label: "数据", value: "—" };
  const byteCount = data[0] ?? 0;
  if (byteCount + 1 === data.length) {
    return { label: "字节数", value: `${byteCount} B` };
  }
  return { label: "数据", value: `${data.length} B` };
}

export function buildDecodedFrame(framing: Framing, raw: readonly number[]): DecodedFrame {
  const bytes = [...raw];

  if (framing === "mbap") {
    const unitId = bytes[6] ?? 0;
    const functionCode = bytes[7] ?? 0;
    const data = bytes.slice(8);
    const payload = decodePayload(data);
    const fields: FrameField[] = [
      {
        label: "事务 ID",
        value: `0x${hex(bytes[0] ?? 0)}${hex(bytes[1] ?? 0)}`,
        offset: 0,
        length: 2,
        tone: "header",
      },
      {
        label: "协议 ID",
        value: `0x${hex(bytes[2] ?? 0)}${hex(bytes[3] ?? 0)}`,
        offset: 2,
        length: 2,
        tone: "header",
      },
      {
        label: "长度",
        value: `${(((bytes[4] ?? 0) << 8) | (bytes[5] ?? 0)).toString()} B`,
        offset: 4,
        length: 2,
        tone: "header",
      },
      { label: "从站 ID", value: unitId.toString(), offset: 6, length: 1, tone: "unit" },
      { label: "功能码", value: `0x${hex(functionCode)}`, offset: 7, length: 1, tone: "function" },
      { label: payload.label, value: payload.value, offset: 8, length: data.length, tone: "data" },
    ];
    return { slaveId: unitId, functionCode, fields };
  }

  if (framing === "ascii") {
    const ascii = new TextDecoder().decode(new Uint8Array(bytes));
    return {
      slaveId: 0,
      functionCode: 0,
      ascii,
      fields: [
        { label: "ASCII 帧", value: ascii.trim(), offset: 0, length: bytes.length, tone: "data" },
      ],
    };
  }

  const unitId = bytes[0] ?? 0;
  const functionCode = bytes[1] ?? 0;
  const data = bytes.slice(2, Math.max(bytes.length - 2, 2));
  const payload = decodePayload(data);
  const crcValue =
    bytes.length >= 2
      ? `${hex(bytes[bytes.length - 2] ?? 0)}${hex(bytes[bytes.length - 1] ?? 0)}`
      : "—";
  const fields: FrameField[] = [
    { label: "从站 ID", value: unitId.toString(), offset: 0, length: 1, tone: "unit" },
    { label: "功能码", value: `0x${hex(functionCode)}`, offset: 1, length: 1, tone: "function" },
    {
      label: payload.label,
      value: payload.value,
      offset: 2,
      length: Math.max(data.length, 0),
      tone: "data",
    },
    {
      label: "CRC16",
      value: `0x${crcValue}`,
      offset: Math.max(bytes.length - 2, 0),
      length: 2,
      tone: "checksum",
    },
  ];
  return { slaveId: unitId, functionCode, fields };
}

export function frameToneSegments(frame: DecodedFrame): Array<{
  start: number;
  length: number;
  tone: FrameField["tone"];
  label: string;
}> {
  return frame.fields
    .filter((field) => field.length > 0)
    .map((field) => ({
      start: field.offset,
      length: field.length,
      tone: field.tone,
      label: field.label,
    }));
}
