// The English catalogue. zh-CN and zh-TW follow with the same keys after the first screen is approved
// (docs/design-language.md, "Approval gates").
export const en = {
  'app.name': 'Zpaste',
  'nav.label': 'Zpaste',
  'paste.untitled': 'Paste {id}',
  'paste.created': 'Created',
  'paste.expires': 'Expires',
  'paste.never': 'Never expires',
  'paste.language': 'Language',
  'paste.plainText': 'Plain text',
  'paste.detailSeparator': ' · ',
  'action.raw': 'Raw',
  'action.copy': 'Copy',
  'action.copied': 'Copied',
  'action.copyFailed': 'Copy failed',
  'action.copiedAnnouncement': 'Paste copied to the clipboard',
  'action.download': 'Download',
  'action.actions': 'Paste actions',
  'outline.label': 'On this page',
  'outline.open': 'Outline',
} as const;

export type MessageKey = keyof typeof en;
