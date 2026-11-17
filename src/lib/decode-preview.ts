import { applyByteOrder, wordsToDataBytes } from "@/lib/demo-codec";
import type { ByteOrder, RegisterValue } from "@/types/models";

const ORDERS: ByteOrder[] = ["abcd", "dcba", "badc", "cdab"];

type CandidateType = "u16" | "i16" | "u32" | "i32" | "f32" | "u64" | "i64" | "f64";

interface CandidateSpec {
  type: CandidateType;
  label: string;
  words: 1 | 2 | 4;
}

const SPECS: CandidateSpec[] = [
  { type: "u16", label: "UINT16", words: 1 },
  { type: "i16", label: "INT16", words: 1 },
  { type: "u32", label: "UINT32", words: 2 },
  { type: "i32", label: "INT32", words: 2 },
  { type: "f32", label: "FLOAT32", words: 2 },
  { type: "u64", label: "UINT64", words: 4 },
  { type: "i64", label: "INT64", words: 4 },
  { type: "f64", label: "FLOAT64", words: 4 },
];

export interface ParseCandidate {
  dataType: CandidateType;
  label: string;
  byteOrder: ByteOrder;
  value: string;
}

function swapWordBytes(word: number): number {
  return ((word & 0xff) << 8) | ((word >> 8) & 0xff);
}

function rawWords(values: readonly RegisterValue[]): Map<number, number> {
  const words = new Map<number, number>();
  for (const value of values) {
    value.words.forEach((word, index) => {
      words.set(value.address + index, word);
    });
  }
  return words;
}

function orderedWords(source: readonly number[], order: ByteOrder): number[] | null {
  if (source.length === 1) {
    const word = source[0];
    if (word === undefined) return null;
    return [order === "badc" || order === "dcba" ? swapWordBytes(word) : word];
  }
  return applyByteOrder(source, order);
}

function formatValue(view: DataView, type: CandidateType): string | null {
  switch (type) {
    case "u16":
      return view.getUint16(0, false).toString();
    case "i16":
      return view.getInt16(0, false).toString();
    case "u32":
      return view.getUint32(0, false).toString();
    case "i32":
      return view.getInt32(0, false).toString();
    case "u64":
      return view.getBigUint64(0, false).toString();
    case "i64":
      return view.getBigInt64(0, false).toString();
    case "f32":
    case "f64": {
      const value = type === "f32" ? view.getFloat32(0, false) : view.getFloat64(0, false);
      if (!Number.isFinite(value)) return null;
      const magnitude = Math.abs(value);
      if (magnitude !== 0 && (magnitude >= 1e9 || magnitude < 1e-4)) {
        return value.toExponential(4);
      }
      return new Intl.NumberFormat("en-US", {
        maximumFractionDigits: 6,
      }).format(value);
    }
    default:
      return null;
  }
}

export function parseCandidates(
  values: readonly RegisterValue[],
  address: number,
): ParseCandidate[] {
  const words = rawWords(values);
  const result: ParseCandidate[] = [];
  for (const spec of SPECS) {
    const source = Array.from({ length: spec.words }, (_, index) => words.get(address + index));
    if (source.some((word) => word === undefined)) continue;
    const seen = new Set<string>();
    for (const order of ORDERS) {
      const ordered = orderedWords(source as number[], order);
      if (!ordered) continue;
      const bytes = wordsToDataBytes(ordered);
      const view = new DataView(Uint8Array.from(bytes).buffer);
      const value = formatValue(view, spec.type);
      if (value === null || seen.has(value)) continue;
      seen.add(value);
      result.push({
        dataType: spec.type,
        label: spec.label,
        byteOrder: order,
        value,
      });
    }
  }
  return result;
}
