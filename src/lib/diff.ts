export type DiffKind = "same" | "add" | "remove";

export interface DiffLine {
  kind: DiffKind;
  text: string;
  leftNo?: number;
  rightNo?: number;
}

const MAX_LINES = 4000;

export function diffLines(leftText: string, rightText: string): DiffLine[] {
  const left = leftText.split(/\r?\n/);
  const right = rightText.split(/\r?\n/);
  if (left.length > MAX_LINES || right.length > MAX_LINES) {
    throw new Error("diffTooLarge");
  }

  const rows = left.length + 1;
  const cols = right.length + 1;
  const table = new Uint16Array(rows * cols);
  for (let i = left.length - 1; i >= 0; i -= 1) {
    for (let j = right.length - 1; j >= 0; j -= 1) {
      table[i * cols + j] =
        left[i] === right[j]
          ? table[(i + 1) * cols + j + 1] + 1
          : Math.max(table[(i + 1) * cols + j], table[i * cols + j + 1]);
    }
  }

  const result: DiffLine[] = [];
  let i = 0;
  let j = 0;
  let leftNo = 1;
  let rightNo = 1;
  while (i < left.length && j < right.length) {
    if (left[i] === right[j]) {
      result.push({ kind: "same", text: left[i], leftNo, rightNo });
      i += 1;
      j += 1;
      leftNo += 1;
      rightNo += 1;
    } else if (table[(i + 1) * cols + j] >= table[i * cols + j + 1]) {
      result.push({ kind: "remove", text: left[i], leftNo });
      i += 1;
      leftNo += 1;
    } else {
      result.push({ kind: "add", text: right[j], rightNo });
      j += 1;
      rightNo += 1;
    }
  }
  while (i < left.length) {
    result.push({ kind: "remove", text: left[i], leftNo });
    i += 1;
    leftNo += 1;
  }
  while (j < right.length) {
    result.push({ kind: "add", text: right[j], rightNo });
    j += 1;
    rightNo += 1;
  }
  return result;
}

export function diffStats(lines: DiffLine[]): { added: number; removed: number; same: number } {
  return lines.reduce(
    (stats, line) => {
      if (line.kind === "add") stats.added += 1;
      else if (line.kind === "remove") stats.removed += 1;
      else stats.same += 1;
      return stats;
    },
    { added: 0, removed: 0, same: 0 },
  );
}
