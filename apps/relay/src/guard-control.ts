import { parseCidr, type Cidr } from "./cidr";
import { isStrikeWhy, parseManual, parseRules, RULE_TEXT, type Guard, type StrikeWhy } from "./guard";
import { MAX_CONN } from "./envelope";

export interface GuardRoom {
  guard: Guard;
  strikeConn(conn: number, why: StrikeWhy): void;
  markConnGood(conn: number): void;
  sendList(): void;
  closeMatching(): void;
}

function target(fields: Record<string, unknown>): Cidr | "all" | null {
  if (fields.all === true) return "all";
  return typeof fields.c === "string" && RULE_TEXT.test(fields.c) ? parseCidr(fields.c) : null;
}

export function guardControl(room: GuardRoom, fields: Record<string, unknown>, nowSec: number): boolean {
  switch (fields.t) {
    case "strike": {
      const { conn, why } = fields;
      if (typeof conn !== "number" || !Number.isInteger(conn) || conn < 1 || conn > MAX_CONN || !isStrikeWhy(why)) return true;
      room.strikeConn(conn, why);
      return true;
    }
    case "good": {
      const { conn } = fields;
      if (typeof conn === "number" && Number.isInteger(conn) && conn >= 1 && conn <= MAX_CONN) room.markConnGood(conn);
      return true;
    }
    case "block.list":
      room.sendList();
      return true;
    case "block.remove": {
      const t = target(fields);
      if (t === null) return true;
      room.guard.remove(t);
      room.sendList();
      return true;
    }
    case "block.set": {
      const list = parseManual(fields.list, nowSec);
      if (list === null) return true;
      room.guard.setManual(list);
      room.closeMatching();
      return true;
    }
    case "trust.set": {
      const list = parseRules(fields.list);
      if (list !== null) room.guard.setTrust(list);
      return true;
    }
    default:
      return false;
  }
}
