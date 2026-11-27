import { describe, expect, it } from "vitest";
import { formatSql } from "./sql-format";

describe("sql formatter", () => {
  it("puts major clauses on their own lines", () => {
    const formatted = formatSql(
      "select id,name from users where active=1 and role='admin' order by name;",
    );
    expect(formatted.split("\n")).toEqual([
      "SELECT",
      "id, name",
      "FROM",
      "users",
      "WHERE",
      "  active=1",
      "  AND role='admin'",
      "ORDER BY",
      "name;",
    ]);
  });

  it("keeps strings untouched", () => {
    const formatted = formatSql("select * from t where name='a,b and c';");
    expect(formatted).toContain("name='a,b and c'");
  });

  it("indents nested parentheses", () => {
    const formatted = formatSql("select * from t where id in (select id from x);");
    expect(formatted).toContain("(\n  SELECT");
  });
});
