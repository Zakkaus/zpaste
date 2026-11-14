import {useState, useSyncExternalStore} from 'react';
import {Button} from '@astryxdesign/core/Button';
import {Icon} from '@astryxdesign/core/Icon';
import {BUTTON} from '@gentoozh/astryx-theme/scale';
import {t} from '../i18n/messages';
import {Moon, Sun} from './icons';
import {appliedChoice, applyChoice, DARK_QUERY, nextChoice, resolveMode, type ColorMode} from './themeMode';

function subscribeSystem(onChange: () => void) {
  const query = window.matchMedia(DARK_QUERY);
  query.addEventListener('change', onChange);
  return () => query.removeEventListener('change', onChange);
}

/** The OS's mode, followed live: a page left open follows an OS switch while it has no stored choice. */
function useSystemMode(): ColorMode {
  return useSyncExternalStore(
    subscribeSystem,
    () => (window.matchMedia(DARK_QUERY).matches ? 'dark' : 'light'),
    () => 'light',
  );
}

/**
 * The top bar's light/dark switch, gentoozh.org's pattern: one icon button that names the mode it
 * switches to, with a moon in light mode and a sun in dark mode. Pressing it on the OS's own mode goes
 * back to following the OS (themeMode.ts, `nextChoice`).
 */
export function ThemeToggle() {
  const system = useSystemMode();
  const [choice, setChoice] = useState(() => appliedChoice(document.documentElement));
  const label = resolveMode(choice, system) === 'dark' ? t('theme.toLight') : t('theme.toDark');
  return (
    <Button
      variant="ghost"
      size={BUTTON.toolbar}
      isIconOnly
      label={label}
      tooltip={label}
      icon={<Icon icon={resolveMode(choice, system) === 'dark' ? Sun : Moon} />}
      onClick={() => {
        const next = nextChoice(choice, system);
        applyChoice(document.documentElement, next);
        setChoice(next);
      }}
    />
  );
}
