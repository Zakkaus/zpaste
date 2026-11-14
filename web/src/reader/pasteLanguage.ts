// The script of a paste's Chinese text, so the page can mark the paste's language and the browser draws
// Traditional text with Traditional glyph forms (docs/design-language.md, "Chinese text"). The page's
// interface language says nothing about the paste's, so this reads the text itself.

/**
 * High-frequency characters that differ between the two scripts, as pairs: the n-th Traditional
 * character simplifies to the n-th Simplified one. Each character belongs to one script only, checked
 * with OpenCC: t2s maps each Traditional one to its pair and s2t leaves it unchanged, while s2t changes
 * every Simplified one.
 */
const TRADITIONAL =
  '這個說們為時會來對與開關體書從過還進國學實現發經機動長問間點東車門見話電樣頭聽寫讀認識讓給號錢買賣處種總應該當沒變員題樂' +
  '歡華區線網頁碼檔設計輸選擇鍵庫報錯誤測試執語標簡單雙戶腦腳統環節請謝歲鐘業產務層際隊陽陰傳備價優億倉勞勢參雖嗎嚴團圖圓場' +
  '塊壓壞夢審導屬島幫帶師幣彈彙徑態憶懷戰據掃換擊數斷晝暫條極構槍樓檢權歐歷歸殺氣決減溫滿漢濟灣無爭爾牆狀獨獲畫疊盡監盤確礎' +
  '禮離稱積穩筆簽糧紀約級紅純紙細終組結絕維綠緒編緣練縣績織繼續罷習聞聯聲肅脫舊艦莊葉補規視親覺覽觀訊記許訪證評詞詳誰課調談' +
  '論講護負財貨質費資賽購贊軍軟載較輕轉辦農連運達遠適遺邊郵鄉醫針鋼錄鏡鐵閱陳陸險隨隱難雜雞靈靜響頂項順須預領頻額顏類顯風飛' +
  '飯館馬驗驚鬧魚鳥麗黃齊齒龍';
const SIMPLIFIED =
  '这个说们为时会来对与开关体书从过还进国学实现发经机动长问间点东车门见话电样头听写读认识让给号钱买卖处种总应该当没变员题乐' +
  '欢华区线网页码档设计输选择键库报错误测试执语标简单双户脑脚统环节请谢岁钟业产务层际队阳阴传备价优亿仓劳势参虽吗严团图圆场' +
  '块压坏梦审导属岛帮带师币弹汇径态忆怀战据扫换击数断昼暂条极构枪楼检权欧历归杀气决减温满汉济湾无争尔墙状独获画叠尽监盘确础' +
  '礼离称积稳笔签粮纪约级红纯纸细终组结绝维绿绪编缘练县绩织继续罢习闻联声肃脱旧舰庄叶补规视亲觉览观讯记许访证评词详谁课调谈' +
  '论讲护负财货质费资赛购赞军软载较轻转办农连运达远适遗边邮乡医针钢录镜铁阅陈陆险随隐难杂鸡灵静响顶项顺须预领频额颜类显风飞' +
  '饭馆马验惊闹鱼鸟丽黄齐齿龙';

/** +1 for a Traditional-only character, -1 for a Simplified-only one. */
const FORMS: ReadonlyMap<number, 1 | -1> = new Map([
  ...[...TRADITIONAL].map(char => [char.charCodeAt(0), 1] as const),
  ...[...SIMPLIFIED].map(char => [char.charCodeAt(0), -1] as const),
]);

/** Only the start of a paste is read: enough to decide, and the cost stays flat for a large paste. */
export const SCAN_LIMIT = 20_000;

/**
 * Below this share of Han characters among Han and Latin letters, the paste is not Chinese text: an
 * English paste quoting a Chinese word keeps the page's language, so a screen reader does not read the
 * whole paste with a Chinese voice. Twenty letters are about four English words.
 */
const LATIN_PER_HAN = 20;

export type PasteLanguage = 'zh-Hant' | 'zh-Hans';

/**
 * `zh-Hant` when the paste's Chinese leans Traditional, `zh-Hans` when it leans Simplified, or
 * undefined (keep the page's language) when it has no Chinese, is mostly Latin text or Japanese (kana
 * beside the kanji), or uses as many characters of each script.
 */
export function pasteLanguage(text: string): PasteLanguage | undefined {
  let han = 0;
  let latin = 0;
  let kana = 0;
  let lean = 0;
  const end = Math.min(text.length, SCAN_LIMIT);
  for (let index = 0; index < end; index++) {
    const code = text.charCodeAt(index);
    if (code < 0x80) {
      const lower = code | 0x20;
      if (lower >= 0x61 && lower <= 0x7a) latin++;
    } else if (code >= 0x3040 && code <= 0x30ff) {
      kana++;
    } else if ((code >= 0x3400 && code <= 0x9fff) || (code >= 0xf900 && code <= 0xfaff)) {
      han++;
      lean += FORMS.get(code) ?? 0;
    }
  }
  if (han === 0 || latin > han * LATIN_PER_HAN || kana * 4 >= han) return undefined;
  return lean > 0 ? 'zh-Hant' : lean < 0 ? 'zh-Hans' : undefined;
}
