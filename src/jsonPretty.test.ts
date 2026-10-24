import { describe, expect, it } from "vitest";
import { MAX_CANDIDATES, MAX_DEPTH, MAX_MESSAGE_LENGTH, findJson, prettyPrint } from "./jsonPretty";

describe("findJson (261022-small-features U2:BR5.1, review R-03)", () => {
  it("finds a message that is one JSON object or array as a whole", () => {
    const object = findJson('{"a":1,"b":[1,2]}');
    expect(object).not.toBeNull();
    expect(object?.prefix).toBe("");
    expect(object?.json).toBe('{"a":1,"b":[1,2]}');
    expect(object?.suffix).toBe("");
    expect(object?.start).toBe(0);
    expect(object?.end).toBe('{"a":1,"b":[1,2]}'.length);
    const array = findJson('[{"a":1}]');
    expect(array?.json).toBe('[{"a":1}]');
    expect(array?.prefix).toBe("");
    expect(array?.suffix).toBe("");
  });

  it("keeps a prefix as it is, its trailing space included", () => {
    const found = findJson('2026-10-24T01:51:32Z abc-123 INFO {"a":1}');
    expect(found?.prefix).toBe("2026-10-24T01:51:32Z abc-123 INFO ");
    expect(found?.json).toBe('{"a":1}');
    expect(found?.suffix).toBe("");
  });

  it("leaves what follows the JSON in the suffix and does not scan it", () => {
    const found = findJson('{"a":1} tail {"b":2}');
    expect(found?.json).toBe('{"a":1}');
    expect(found?.suffix).toBe(' tail {"b":2}');
    expect(findJson('{"a":1}\n')?.suffix).toBe("\n");
  });

  it("finds nothing in an unclosed, broken or lone brace", () => {
    expect(findJson('{"a":1,')).toBeNull();
    expect(findJson('{"a":}')).toBeNull();
    expect(findJson("{")).toBeNull();
    expect(findJson("[")).toBeNull();
    expect(findJson('{"a":1,}')).toBeNull();
    expect(findJson("[1,]")).toBeNull();
    expect(findJson("{a:1}")).toBeNull();
    expect(findJson('{"a":01}')).toBeNull();
    expect(findJson('{"a":1.}')).toBeNull();
    expect(findJson('{"a":tru}')).toBeNull();
    expect(findJson('{"a":"\\x"}')).toBeNull();
    expect(findJson('{"a":"\\u12G4"}')).toBeNull();
  });

  it("tries the next candidate when the first cannot be read to its end", () => {
    const found = findJson('{oops} {"a":1}');
    expect(found?.prefix).toBe("{oops} ");
    expect(found?.json).toBe('{"a":1}');
    const nested = findJson('[broken {"ok":true} more');
    expect(nested?.prefix).toBe("[broken ");
    expect(nested?.json).toBe('{"ok":true}');
    expect(nested?.suffix).toBe(" more");
  });

  it("skips an array of scalars only and takes the object after it (R-03)", () => {
    const found = findJson('[1234] {"event":"x"}');
    expect(found?.prefix).toBe("[1234] ");
    expect(found?.json).toBe('{"event":"x"}');
    expect(findJson('[200] ["a", null] {"b":2}')?.json).toBe('{"b":2}');
    expect(findJson("[1234]")).toBeNull();
    expect(findJson("[]")).toBeNull();
    // An array holding an object or an array is a candidate.
    expect(findJson('[{"a":1}]')?.json).toBe('[{"a":1}]');
    expect(findJson("[[1]]")?.json).toBe("[[1]]");
    expect(findJson("[1, [2]]")?.json).toBe("[1, [2]]");
  });

  it("reads braces and quotes inside strings as string content", () => {
    const text = '{"s":"a{b\\"c"}';
    const found = findJson(text);
    expect(found?.json).toBe(text);
    expect(found?.suffix).toBe("");
    expect(findJson('{"s":"}"} x')?.json).toBe('{"s":"}"}');
    expect(findJson('{"s":"\\\\"}')?.json).toBe('{"s":"\\\\"}');
    expect(findJson('{"u":"\\u00e9\\n\\t\\/"}')?.json).toBe('{"u":"\\u00e9\\n\\t\\/"}');
    // A `{` inside a string of a candidate that fails is still tried as its
    // own candidate later, and fails on its own.
    expect(findJson('{"s":"{"')).toBeNull();
  });

  it("finds nothing in a message without JSON or an empty one", () => {
    expect(findJson("")).toBeNull();
    expect(findJson("plain text without braces")).toBeNull();
    expect(findJson("   ")).toBeNull();
    expect(findJson("1234")).toBeNull();
    expect(findJson('"a string"')).toBeNull();
  });

  it("gives up beyond the depth, candidate and length limits", () => {
    expect(MAX_DEPTH).toBe(256);
    expect(MAX_CANDIDATES).toBe(64);
    expect(MAX_MESSAGE_LENGTH).toBe(1_048_576);
    const deep = "[".repeat(300) + "]".repeat(300);
    expect(findJson(deep)).toBeNull();
    const justFits = "[".repeat(256) + "]".repeat(256);
    expect(findJson(justFits)?.json).toBe(justFits);
    const tooDeep = "[".repeat(257) + "]".repeat(257);
    expect(findJson(tooDeep)).toBeNull();
    // 65 candidates: the 65th (readable) one is never tried.
    expect(findJson(`${"{ ".repeat(65)}{"a":1}`)).toBeNull();
    expect(findJson(`${"{ ".repeat(63)}{"a":1}`)?.json).toBe('{"a":1}');
    const huge = `{"a":1}${" ".repeat(MAX_MESSAGE_LENGTH)}`;
    expect(huge.length).toBeGreaterThan(MAX_MESSAGE_LENGTH);
    expect(findJson(huge)).toBeNull();
    const maximal = `{"a":1}${" ".repeat(MAX_MESSAGE_LENGTH - 7)}`;
    expect(maximal.length).toBe(MAX_MESSAGE_LENGTH);
    expect(findJson(maximal)?.json).toBe('{"a":1}');
  });

  it("returns a prefix, json and suffix that rebuild the message", () => {
    for (const message of [
      'INFO {"a":1} done',
      '{"a":[1,{"b":null}]}',
      'x [1] {"k":"v"} y',
      '  {"only":true}  ',
    ]) {
      const found = findJson(message);
      expect(found).not.toBeNull();
      expect(found!.prefix + found!.json + found!.suffix).toBe(message);
      expect(message.slice(found!.start, found!.end)).toBe(found!.json);
    }
  });
});

describe("prettyPrint (261022-small-features U2:BR5.2)", () => {
  it("keeps every token as written: numbers, strings, keys, literals and their order", () => {
    const json =
      '{"n":1.0,"e":1e5,"z":-0,"big":12345678901234567890,"u":"é\\n","t":true,"f":false,"x":null}';
    expect(prettyPrint(json)).toBe(
      [
        "{",
        '  "n": 1.0,',
        '  "e": 1e5,',
        '  "z": -0,',
        '  "big": 12345678901234567890,',
        '  "u": "é\\n",',
        '  "t": true,',
        '  "f": false,',
        '  "x": null',
        "}",
      ].join("\n"),
    );
    expect(prettyPrint('{"b":1,"a":2}')).toBe('{\n  "b": 1,\n  "a": 2\n}');
    expect(prettyPrint('{"s":"a{b\\"c"}')).toBe('{\n  "s": "a{b\\"c"\n}');
  });

  it("indents with two spaces, one element per line, and keeps empty containers", () => {
    expect(prettyPrint('{"a":{"b":[1,2,{"c":[]}]},"d":{}}')).toBe(
      [
        "{",
        '  "a": {',
        '    "b": [',
        "      1,",
        "      2,",
        "      {",
        '        "c": []',
        "      }",
        "    ]",
        "  },",
        '  "d": {}',
        "}",
      ].join("\n"),
    );
    expect(prettyPrint("[]")).toBe("[]");
    expect(prettyPrint("{}")).toBe("{}");
    expect(prettyPrint("[[1]]")).toBe("[\n  [\n    1\n  ]\n]");
  });

  it("drops the original whitespace and line breaks", () => {
    expect(prettyPrint('{ "a" : 1 ,\n\t"b":\r\n[ 1 , 2 ] }')).toBe(
      '{\n  "a": 1,\n  "b": [\n    1,\n    2\n  ]\n}',
    );
    expect(prettyPrint('{\n  "a": 1\n}')).toBe('{\n  "a": 1\n}');
  });

  it("pretty-prints what findJson found", () => {
    const found = findJson('INFO {"a":{"b":1}} tail');
    expect(found?.pretty).toBe('{\n  "a": {\n    "b": 1\n  }\n}');
    expect(found!.prefix + found!.pretty + found!.suffix).toBe(
      'INFO {\n  "a": {\n    "b": 1\n  }\n} tail',
    );
  });
});
