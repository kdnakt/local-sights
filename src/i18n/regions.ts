/**
 * Region names of the region picker (261020-carryover-fixes FR5.1, FR5.1.1).
 *
 * The Rust core sends the region options as codes only (`SessionView.regions`,
 * `catalog/regions.rs`); the names are screen texts, so they live here with
 * the other texts of each language. Every region of the core's built-in list
 * `STANDARD_REGIONS` is named, in English and in Japanese, as the AWS
 * documentation's list of Regions writes it ("AWS Regions",
 * docs.aws.amazon.com/global-infrastructure/latest/regions/aws-regions.html
 * and its ja_jp page, checked 2026-10-21; half-width parentheses in both).
 * regions.test.ts checks that the table names exactly the regions of that
 * list (FR5.10). A region the table does not name (a profile's default
 * region missing from the built-in list, FR5.5) is shown by its code alone.
 * Nothing is fetched from the network (NFR5).
 */

import type { Locale } from "./messages";

/** The English and Japanese name of a region. */
export interface RegionName {
  en: string;
  ja: string;
}

/** The name of every region of `STANDARD_REGIONS`, by code. */
export const REGION_NAMES: Readonly<Record<string, RegionName>> = {
  "af-south-1": { en: "Africa (Cape Town)", ja: "アフリカ (ケープタウン)" },
  "ap-east-1": { en: "Asia Pacific (Hong Kong)", ja: "アジアパシフィック (香港)" },
  "ap-east-2": { en: "Asia Pacific (Taipei)", ja: "アジアパシフィック (台北)" },
  "ap-northeast-1": { en: "Asia Pacific (Tokyo)", ja: "アジアパシフィック (東京)" },
  "ap-northeast-2": { en: "Asia Pacific (Seoul)", ja: "アジアパシフィック (ソウル)" },
  "ap-northeast-3": { en: "Asia Pacific (Osaka)", ja: "アジアパシフィック (大阪)" },
  "ap-south-1": { en: "Asia Pacific (Mumbai)", ja: "アジアパシフィック (ムンバイ)" },
  "ap-south-2": { en: "Asia Pacific (Hyderabad)", ja: "アジアパシフィック (ハイデラバード)" },
  "ap-southeast-1": { en: "Asia Pacific (Singapore)", ja: "アジアパシフィック (シンガポール)" },
  "ap-southeast-2": { en: "Asia Pacific (Sydney)", ja: "アジアパシフィック (シドニー)" },
  "ap-southeast-3": { en: "Asia Pacific (Jakarta)", ja: "アジアパシフィック (ジャカルタ)" },
  "ap-southeast-4": { en: "Asia Pacific (Melbourne)", ja: "アジアパシフィック (メルボルン)" },
  "ap-southeast-5": { en: "Asia Pacific (Malaysia)", ja: "アジアパシフィック (マレーシア)" },
  "ap-southeast-6": {
    en: "Asia Pacific (New Zealand)",
    ja: "アジアパシフィック (ニュージーランド)",
  },
  "ap-southeast-7": { en: "Asia Pacific (Thailand)", ja: "アジアパシフィック (タイ)" },
  "ca-central-1": { en: "Canada (Central)", ja: "カナダ (中部)" },
  "ca-west-1": { en: "Canada West (Calgary)", ja: "カナダ西部 (カルガリー)" },
  "eu-central-1": { en: "Europe (Frankfurt)", ja: "欧州 (フランクフルト)" },
  "eu-central-2": { en: "Europe (Zurich)", ja: "欧州 (チューリッヒ)" },
  "eu-north-1": { en: "Europe (Stockholm)", ja: "欧州 (ストックホルム)" },
  "eu-south-1": { en: "Europe (Milan)", ja: "欧州 (ミラノ)" },
  "eu-south-2": { en: "Europe (Spain)", ja: "欧州 (スペイン)" },
  "eu-west-1": { en: "Europe (Ireland)", ja: "欧州 (アイルランド)" },
  "eu-west-2": { en: "Europe (London)", ja: "欧州 (ロンドン)" },
  "eu-west-3": { en: "Europe (Paris)", ja: "欧州 (パリ)" },
  "il-central-1": { en: "Israel (Tel Aviv)", ja: "イスラエル (テルアビブ)" },
  "me-central-1": { en: "Middle East (UAE)", ja: "中東 (アラブ首長国連邦)" },
  "me-south-1": { en: "Middle East (Bahrain)", ja: "中東 (バーレーン)" },
  "mx-central-1": { en: "Mexico (Central)", ja: "メキシコ (中部)" },
  "sa-east-1": { en: "South America (São Paulo)", ja: "南米 (サンパウロ)" },
  "us-east-1": { en: "US East (N. Virginia)", ja: "米国東部 (バージニア北部)" },
  "us-east-2": { en: "US East (Ohio)", ja: "米国東部 (オハイオ)" },
  "us-west-1": { en: "US West (N. California)", ja: "米国西部 (北カリフォルニア)" },
  "us-west-2": { en: "US West (Oregon)", ja: "米国西部 (オレゴン)" },
};

/** The names of `code`, or `undefined` when the table does not name it. */
function namesOf(code: string): RegionName | undefined {
  return Object.prototype.hasOwnProperty.call(REGION_NAMES, code) ? REGION_NAMES[code] : undefined;
}

/**
 * FR5.3, FR5.5, FR5.7: how the picker writes `code`: the code and its name in
 * the screen's language (`ap-northeast-1 アジアパシフィック (東京)`), or the
 * code alone when the table does not name it.
 */
export function regionLabel(code: string, locale: Locale): string {
  const names = namesOf(code);
  return names === undefined ? code : `${code} ${names[locale]}`;
}

/** A text as the filter compares it: lower case, full-width parentheses as half-width (A7). */
function normalized(text: string): string {
  return text.replace(/（/g, "(").replace(/）/g, ")").toLowerCase();
}

/**
 * FR5.4, FR5.5: the codes of `codes` whose code, English name or Japanese
 * name holds `text` (on either screen, whatever the case, full-width and
 * half-width parentheses alike, A7), in their order. An empty or blank text
 * keeps them all.
 */
export function filterRegionOptions(codes: readonly string[], text: string): string[] {
  const wanted = normalized(text.trim());
  if (wanted === "") {
    return [...codes];
  }
  return codes.filter((code) => {
    const names = namesOf(code);
    const texts = names === undefined ? [code] : [code, names.en, names.ja];
    return texts.some((candidate) => normalized(candidate).includes(wanted));
  });
}
