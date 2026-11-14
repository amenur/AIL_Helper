import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readingMultiple, wps } from './format.ts';

test('wps traduce tokens/s a palabras/s legibles', () => {
  assert.equal(wps(0), '—');
  assert.equal(wps(10), '~7,0');
  assert.equal(wps(118), '~85');
  assert.ok(Math.abs(readingMultiple(5.714) - 1) < 0.01);
});
