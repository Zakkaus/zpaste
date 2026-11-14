import {Fragment, useState, type ReactNode} from 'react';
import * as stylex from '@stylexjs/stylex';
import {Button} from '@astryxdesign/core/Button';
import {useAnnounce, useClipboard} from '@astryxdesign/core/hooks';
import {HStack, VStack} from '@astryxdesign/core/Layout';
import {Heading} from '@astryxdesign/core/Heading';
import {Text} from '@astryxdesign/core/Text';
import {Timestamp} from '@astryxdesign/core/Timestamp';
import {BUTTON, SPACE} from '@gentoozh/astryx-theme/scale';
import type {EmbeddedPaste} from '../shell/embedded';
import {t} from '../i18n/messages';
import {layout} from '../theme/layout.stylex';
import {languageName} from './languages';

const styles = stylex.create({
  wrap: {flexWrap: 'wrap'},
  // The title row shrinks before the actions do, and a long title wraps instead of widening the page.
  title: {flexBasis: 0, flexGrow: 1, minWidth: layout.titleMinWidth, overflowWrap: 'anywhere'},
  // The page's h1 in the section-heading role: one step above body text, one below the paste's own
  // first heading, which carries the page (docs/design-language.md, "Positioning").
  heading: {
    fontSize: 'var(--text-heading-2-size)',
    fontWeight: 'var(--text-heading-2-weight)',
    lineHeight: 'var(--text-heading-2-leading)',
  },
  // A detail never breaks inside its value; the row breaks between details.
  detail: {whiteSpace: 'nowrap'},
  // Copy's visible labels share one grid cell, so the button is as wide as its longest label in every state.
  labels: {display: 'inline-grid', justifyItems: 'center'},
  label: {gridArea: '1 / 1'},
  labelHidden: {visibility: 'hidden'},
});

/**
 * Copy's states: the accessible name, and the visible label, which stays as short as "Copied" so the
 * button keeps the width of its neighbours' kind. The name still says what failed.
 */
const COPY_STATES = {
  copy: {name: 'action.copy', visible: 'action.copy'},
  copied: {name: 'action.copied', visible: 'action.copied'},
  failed: {name: 'action.copyFailed', visible: 'action.copyFailedShort'},
} as const;

/**
 * The page title row: the paste's title and details, with its actions at the end. The title is
 * compact on purpose: the paste's own headings carry the page (docs/design-language.md, "Positioning").
 */
export function PasteHeader({data, extension}: {data: EmbeddedPaste; extension: string | null}) {
  const {paste} = data;
  const fileName = extension == null ? paste.id : `${paste.id}.${extension}`;
  const details: Array<[string, ReactNode]> = [];
  if (paste.created_at != null) {
    details.push([t('paste.created'), <Timestamp value={paste.created_at} format="date" type="inherit" color="inherit" />]);
  }
  details.push([
    t('paste.expires'),
    paste.expires_at == null ? t('paste.never') : <Timestamp value={paste.expires_at} format="date" type="inherit" color="inherit" />,
  ]);
  details.push([t('paste.language'), languageName(extension) ?? t('paste.plainText')]);

  return (
    <HStack gap={SPACE.block} vAlign="center" hAlign="between" width="100%" xstyle={styles.wrap}>
      <VStack gap={SPACE.hairline} xstyle={styles.title}>
        <Heading level={1} xstyle={styles.heading}>
          {paste.title ?? t('paste.untitled', {id: paste.id})}
        </Heading>
        <Text type="supporting" color="secondary">
          {details.map(([label, value], index) => (
            <Fragment key={label}>
              {index > 0 ? t('paste.detailSeparator') : null}
              <span {...stylex.props(styles.detail)}>
                {label} {value}
              </span>
            </Fragment>
          ))}
        </Text>
      </VStack>
      <PasteActions content={data.content} id={paste.id} fileName={fileName} canEdit={data.can_edit === true} />
    </HStack>
  );
}

/**
 * Copy is the view's one primary action; Edit, Raw and Download are secondary buttons of the same size.
 * None carries an icon, as in gentoozh.org's toolbar rows, so the four read as one set; Copy's feedback
 * replaces its label in place without changing its width.
 * Edit comes next to Copy because both act on the paste; Raw and Download open it elsewhere. Edit shows
 * only when the shell says the reader may edit (`can_edit`) and opens `/<id>/edit`, the edit page (P2).
 * Raw and Download use the existing routes (`/dl` names the file after the key, so it keeps the extension);
 * Copy copies the text that is already in the page. A failed copy says so on the button and to a screen
 * reader at once; the hook announces only success.
 */
function PasteActions({content, id, fileName, canEdit}: {content: string; id: string; fileName: string; canEdit: boolean}) {
  const {copy, isCopied} = useClipboard({announce: t('action.copiedAnnouncement')});
  const announce = useAnnounce();
  const [isFailed, setFailed] = useState(false);
  const state = isCopied ? 'copied' : isFailed ? 'failed' : 'copy';
  const onCopy = () =>
    void copy(content)
      .catch(() => false)
      .then(copied => {
        setFailed(!copied);
        if (!copied) announce(t('action.copyFailed'), 'assertive');
      });
  return (
    <HStack gap={SPACE.cluster} role="group" aria-label={t('action.actions')} xstyle={styles.wrap}>
      <Button
        variant="primary"
        size={BUTTON.toolbar}
        label={t(COPY_STATES[state].name)}
        onClick={onCopy}>
        <span {...stylex.props(styles.labels)}>
          {Object.entries(COPY_STATES).map(([key, {visible}]) => (
            <span key={key} aria-hidden {...stylex.props(styles.label, key !== state && styles.labelHidden)}>
              {t(visible)}
            </span>
          ))}
        </span>
      </Button>
      {canEdit ? <Button variant="secondary" size={BUTTON.toolbar} label={t('action.edit')} href={`/${id}/edit`} /> : null}
      <Button variant="secondary" size={BUTTON.toolbar} label={t('action.raw')} href={`/raw/${id}`} />
      <Button variant="secondary" size={BUTTON.toolbar} label={t('action.download')} href={`/dl/${fileName}`} />
    </HStack>
  );
}
