import {Fragment, useState, type ReactNode} from 'react';
import * as stylex from '@stylexjs/stylex';
import {Button} from '@astryxdesign/core/Button';
import {useClipboard} from '@astryxdesign/core/hooks';
import {Icon} from '@astryxdesign/core/Icon';
import {HStack, VStack} from '@astryxdesign/core/Layout';
import {Heading} from '@astryxdesign/core/Heading';
import {Text} from '@astryxdesign/core/Text';
import {Timestamp} from '@astryxdesign/core/Timestamp';
import {BUTTON, SPACE} from '@gentoozh/astryx-theme/scale';
import type {EmbeddedPaste} from '../shell/embedded';
import {t} from '../i18n/messages';
import {languageName} from './languages';

const styles = stylex.create({
  wrap: {flexWrap: 'wrap'},
  // The title row shrinks before the actions do, and a long title wraps instead of widening the page.
  title: {flexBasis: 0, flexGrow: 1, minWidth: 'min(100%, 16rem)', overflowWrap: 'anywhere'},
  // The page's h1 in the section-heading role: one step above body text, one below the paste's own
  // first heading, which carries the page (docs/design-language.md, "Positioning").
  heading: {
    fontSize: 'var(--text-heading-2-size)',
    fontWeight: 'var(--text-heading-2-weight)',
    lineHeight: 'var(--text-heading-2-leading)',
  },
  // A detail never breaks inside its value; the row breaks between details.
  detail: {whiteSpace: 'nowrap'},
});

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
      <PasteActions content={data.content} id={paste.id} fileName={fileName} />
    </HStack>
  );
}

/**
 * Copy is the view's one primary action; Raw and Download are secondary buttons of the same size.
 * Raw and Download use the existing routes (`/dl` names the file after the key, so it keeps the extension);
 * Copy copies the text that is already in the page.
 */
function PasteActions({content, id, fileName}: {content: string; id: string; fileName: string}) {
  const {copy, isCopied} = useClipboard({announce: t('action.copiedAnnouncement')});
  const [isFailed, setFailed] = useState(false);
  const copyLabel = isCopied ? t('action.copied') : isFailed ? t('action.copyFailed') : t('action.copy');
  return (
    <HStack gap={SPACE.cluster} role="group" aria-label={t('action.actions')} xstyle={styles.wrap}>
      <Button
        variant="primary"
        size={BUTTON.toolbar}
        label={copyLabel}
        icon={<Icon icon={isCopied ? 'check' : 'copy'} />}
        onClick={() => void copy(content).then(copied => setFailed(!copied))}
      />
      <Button variant="secondary" size={BUTTON.toolbar} label={t('action.raw')} href={`/raw/${id}`} />
      <Button variant="secondary" size={BUTTON.toolbar} label={t('action.download')} href={`/dl/${fileName}`} />
    </HStack>
  );
}
