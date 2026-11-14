import {describe, expect, it} from 'vitest';
import {MARKDOWN_LIMIT, renderPlan, TEXT_LIMIT} from './renderPlan';

describe('render plan', () => {
  it('draws Markdown up to the Markdown limit and plain text past it', () => {
    expect(renderPlan('a'.repeat(MARKDOWN_LIMIT), true)).toEqual({renderer: 'markdown'});
    const over = 'a'.repeat(MARKDOWN_LIMIT + 1);
    expect(renderPlan(over, true)).toEqual({renderer: 'text', text: over, notice: 'plain'});
  });

  it('draws other text whole up to the text limit, without a notice', () => {
    const text = 'a'.repeat(TEXT_LIMIT);
    expect(renderPlan(text, false)).toEqual({renderer: 'text', text, notice: null});
  });

  it('shows only the start of a paste past the text limit, cut after a line', () => {
    const line = `${'a'.repeat(99)}\n`;
    const content = line.repeat(TEXT_LIMIT / line.length + 10);
    for (const markdown of [true, false]) {
      const plan = renderPlan(content, markdown);
      expect(plan.renderer).toBe('text');
      if (plan.renderer !== 'text') continue;
      expect(plan.notice).toBe('truncated');
      expect(plan.text.length).toBeLessThanOrEqual(TEXT_LIMIT);
      expect(plan.text.endsWith('\n')).toBe(true);
      expect(content.startsWith(plan.text)).toBe(true);
    }
  });

  it('cuts a paste without line breaks at the limit, never inside a surrogate pair', () => {
    const plan = renderPlan(`${'a'.repeat(TEXT_LIMIT - 1)}😀b`, false);
    expect(plan.renderer === 'text' && plan.text).toBe('a'.repeat(TEXT_LIMIT - 1));
    const whole = renderPlan('a'.repeat(TEXT_LIMIT + 5), false);
    expect(whole.renderer === 'text' && whole.text.length).toBe(TEXT_LIMIT);
  });
});
