import {en, type MessageKey} from './en';

export const LOCALE = 'en';

/** The message for `key`, with each `{name}` replaced from `values`. */
export function t(key: MessageKey, values: Record<string, string> = {}): string {
  return en[key].replace(/\{(\w+)\}/g, (match, name: string) => values[name] ?? match);
}
