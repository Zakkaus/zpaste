// A long synthetic Markdown paste for the screenshot script: the plan's 1,600-line document, with an
// outline taller than the desktop rail. Built by code rather than kept as a 1,700-line file;
// fixture.test.ts checks its size.

const AREAS = ['storage', 'retention', 'rendering', 'outline', 'tables', 'search', 'accounts', 'backups', 'limits', 'mirrors'];
const TOPICS = ['Scope', 'Behaviour', 'Checks'];

function section(part: number, sub: number): string[] {
  const area = AREAS[(part + sub) % AREAS.length];
  return [
    `### ${part}.${sub} ${TOPICS[sub - 1]} of ${area}`,
    '',
    `This subsection describes how ${area} behaves in part ${part}. The paragraph is long enough to wrap on`,
    `a phone and on a laptop, so the reading column and the outline both carry real lines. It names a file`,
    `such as \`web/src/reader/${area}.ts\` and an identifier such as \`${area}_limit_${part}\` inline.`,
    '',
    `- The ${area} rule applies to every paste in part ${part}.`,
    `- A change to it is recorded in \`docs/${area}.md\`.`,
    `- The check for it runs in the merge gate.`,
    `- Its limit is ${part * 10 + sub} entries per request.`,
    `- A request over the limit gets a clear error, not a partial result.`,
    '',
    `> A note on ${area}: the limit counts per paste, not per account.`,
    '',
    '```ts',
    `export function ${area}Limit${part}${sub}(count: number): boolean {`,
    `  // Part ${part}, subsection ${sub}.`,
    `  const limit = ${part * 10 + sub};`,
    '  if (count < 0) {',
    '    return false;',
    '  }',
    '  return count <= limit;',
    '}',
    '```',
    '',
    `The last paragraph of ${part}.${sub} closes the subsection with a second short note on ${area}.`,
    '',
  ];
}

/** About 1,700 lines with 81 headings: the `#` title, 20 `##` parts and three `###` sections in each. */
export function longMarkdown(): string {
  const lines = [
    '# Long synthetic paste',
    '',
    'A generated paste for the screenshot script. Its outline is taller than the desktop rail.',
    '',
    '| File | Kind | Note | Owner | Updated |',
    '| --- | --- | --- | --- | --- |',
    '| `web/src/reader/MarkdownView.module.css` | style | table and code rules | web | 2026-11-15 |',
    '| `REQUIREMENTS.md` | rules | numbers, gates and exceptions | all | 2026-11-14 |',
    '| `scripts/check-requirements.py` | gate | fails on an unfilled entry | ci | 2026-11-13 |',
    '| `sha256:9f2c4e7a1b3d5f6089abcdef0123456789abcdef0123456789abcdef01234567` | digest | one piece wider than half the box | build | 2026-11-12 |',
    '',
  ];
  for (let part = 1; part <= 20; part++) {
    lines.push(`## Part ${part}: ${AREAS[part % AREAS.length]}`, '', `Part ${part} groups three subsections.`, '');
    for (let sub = 1; sub <= TOPICS.length; sub++) lines.push(...section(part, sub));
  }
  return lines.join('\n');
}
