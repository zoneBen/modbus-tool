interface FieldSpec {
  min: number;
  max: number;
  values: Set<number>;
  wildcard: boolean;
}

export interface ParsedCron {
  minutes: FieldSpec;
  hours: FieldSpec;
  days: FieldSpec;
  months: FieldSpec;
  weekdays: FieldSpec;
  dayRestricted: boolean;
  weekdayRestricted: boolean;
}

const MONTH_NAMES: Record<string, number> = {
  jan: 1,
  feb: 2,
  mar: 3,
  apr: 4,
  may: 5,
  jun: 6,
  jul: 7,
  aug: 8,
  sep: 9,
  oct: 10,
  nov: 11,
  dec: 12,
};

const WEEKDAY_NAMES: Record<string, number> = {
  sun: 0,
  mon: 1,
  tue: 2,
  wed: 3,
  thu: 4,
  fri: 5,
  sat: 6,
};

function parseField(
  raw: string,
  min: number,
  max: number,
  names?: Record<string, number>,
): FieldSpec {
  const values = new Set<number>();
  let wildcard = false;
  for (const part of raw.split(",")) {
    const segment = part.trim();
    if (!segment) throw new Error("cronEmptyField");
    const [rangePart, stepPart] = segment.split("/");
    const step = stepPart === undefined ? 1 : Number(stepPart);
    if (!Number.isInteger(step) || step <= 0) throw new Error("cronInvalidStep");
    let start: number;
    let end: number;
    if (rangePart === "*" || rangePart === "?") {
      wildcard = true;
      start = min;
      end = max;
    } else if (rangePart.includes("-")) {
      const [left, right] = rangePart.split("-");
      start = parseValue(left, names);
      end = parseValue(right, names);
    } else {
      start = parseValue(rangePart, names);
      end = stepPart === undefined ? start : max;
    }
    if (start < min || end > max || start > end) throw new Error("cronOutOfRange");
    for (let value = start; value <= end; value += step) values.add(value);
  }
  return { min, max, values, wildcard };
}

function parseValue(raw: string, names?: Record<string, number>): number {
  const normalized = raw.trim().toLowerCase();
  if (names && normalized in names) return names[normalized];
  const value = Number(normalized);
  if (!Number.isInteger(value)) throw new Error("cronInvalidField");
  return value;
}

export function parseCron(expression: string): ParsedCron {
  const fields = expression.trim().split(/\s+/);
  if (fields.length !== 5) throw new Error("cronFieldCount");
  const [minute, hour, day, month, weekday] = fields;
  const weekdays = parseField(weekday, 0, 7, WEEKDAY_NAMES);
  if (weekdays.values.has(7)) {
    weekdays.values.delete(7);
    weekdays.values.add(0);
  }
  return {
    minutes: parseField(minute, 0, 59),
    hours: parseField(hour, 0, 23),
    days: parseField(day, 1, 31),
    months: parseField(month, 1, 12, MONTH_NAMES),
    weekdays,
    dayRestricted: day !== "*" && day !== "?",
    weekdayRestricted: weekday !== "*" && weekday !== "?",
  };
}

function matches(parsed: ParsedCron, date: Date): boolean {
  const minute = date.getMinutes();
  const hour = date.getHours();
  const day = date.getDate();
  const month = date.getMonth() + 1;
  const weekday = date.getDay();
  if (!parsed.minutes.values.has(minute)) return false;
  if (!parsed.hours.values.has(hour)) return false;
  if (!parsed.months.values.has(month)) return false;
  const dayMatch = parsed.days.values.has(day);
  const weekdayMatch = parsed.weekdays.values.has(weekday);
  if (parsed.dayRestricted && parsed.weekdayRestricted) return dayMatch || weekdayMatch;
  if (parsed.dayRestricted) return dayMatch;
  if (parsed.weekdayRestricted) return weekdayMatch;
  return true;
}

export function nextRuns(expression: string, count = 5, from = new Date()): Date[] {
  const parsed = parseCron(expression);
  const cursor = new Date(from.getTime());
  cursor.setSeconds(0, 0);
  cursor.setMinutes(cursor.getMinutes() + 1);
  const results: Date[] = [];
  const limit = 366 * 24 * 60 * 2;
  for (let step = 0; step < limit && results.length < count; step += 1) {
    if (matches(parsed, cursor)) results.push(new Date(cursor.getTime()));
    cursor.setMinutes(cursor.getMinutes() + 1);
  }
  if (results.length === 0) throw new Error("cronNoMatches");
  return results;
}

export function validateCron(expression: string): string | null {
  try {
    parseCron(expression);
    return null;
  } catch (error: unknown) {
    return error instanceof Error ? error.message : "cronInvalidField";
  }
}
