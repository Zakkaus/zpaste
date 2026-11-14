import {describe, expect, it} from 'vitest';
import {pasteLanguage, SCAN_LIMIT} from './pasteLanguage';
import HANT from '../../fixtures/chinese-hant.md?raw';
import HANS from '../../fixtures/chinese-hans.md?raw';

describe('paste language', () => {
  it('marks Traditional Chinese as zh-Hant', () => {
    expect(pasteLanguage('這個設定會在下次開機時生效，請先確認網路連線。')).toBe('zh-Hant');
    expect(pasteLanguage(HANT)).toBe('zh-Hant');
  });

  it('marks Simplified Chinese as zh-Hans', () => {
    expect(pasteLanguage('这个设置会在下次开机时生效，请先确认网络连接。')).toBe('zh-Hans');
    expect(pasteLanguage(HANS)).toBe('zh-Hans');
  });

  it('follows the script most of a mixed paste uses', () => {
    expect(pasteLanguage(`${HANT}\n\n这个设置会生效。`)).toBe('zh-Hant');
    expect(pasteLanguage(`${HANS}\n\n這個設定會生效。`)).toBe('zh-Hans');
  });

  it('keeps the page language when the scripts are even or the text tells them apart nowhere', () => {
    expect(pasteLanguage('這个')).toBeUndefined();
    expect(pasteLanguage('中文文本')).toBeUndefined();
  });

  it('needs several characters of one script and a clear lead before it marks a paste', () => {
    // Japanese in kanji alone: 設 and 項 are Traditional forms, but two are not evidence.
    expect(pasteLanguage('設定項目')).toBeUndefined();
    expect(pasteLanguage('設定項目一覧')).toBeUndefined();
    expect(pasteLanguage('這個說們')).toBeUndefined();
    expect(pasteLanguage('這個說們為')).toBe('zh-Hant');
    // Five against three is no clear lead; six against three is.
    expect(pasteLanguage('這個說們為这个说')).toBeUndefined();
    expect(pasteLanguage('這個說們為時这个说')).toBe('zh-Hant');
  });

  it('keeps the page language for English, code and Japanese', () => {
    expect(pasteLanguage('# Release checklist\n\nTag the release once every gate passes.')).toBeUndefined();
    expect(pasteLanguage('fn main() {\n    println!("{}", 1 + 2);\n}\n')).toBeUndefined();
    expect(pasteLanguage(`${'Run the build and read the log before you tag. '.repeat(40)}這個`)).toBeUndefined();
    expect(pasteLanguage('この設定は次の起動時に有効になります。')).toBeUndefined();
  });

  it('reads only the start of a long paste', () => {
    expect(pasteLanguage(`${'這個'.repeat(SCAN_LIMIT / 2)}${'这个'.repeat(SCAN_LIMIT)}`)).toBe('zh-Hant');
  });
});
