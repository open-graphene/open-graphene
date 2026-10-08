import test from 'node:test';
import assert from 'node:assert/strict';
import { isHeadFresh, isSameHead } from '../graphene-primitives/dist/index.js';

const now = Date.UTC(2026, 9, 8, 12);
const limits = {
  maxHeadAgeSeconds: 30,
  maxHeadTimeAheadSeconds: 5,
};

function head(offsetSeconds = 0) {
  return {
    head_block_number: 100,
    head_block_id: new Uint8Array(20).fill(1),
    time: new Date(now + offsetSeconds * 1000).toISOString().slice(0, 19),
  };
}

for (const [offset, expected] of [
  [-31, false],
  [-30, true],
  [0, true],
  [5, true],
  [6, false],
]) {
  test(`head freshness at offset ${offset} seconds is ${expected}`, (t) => {
    t.mock.method(Date, 'now', () => now);
    assert.equal(isHeadFresh(head(offset), limits), expected);
  });
}

test('freshness preserves subsecond local clock precision', (t) => {
  t.mock.method(Date, 'now', () => now + 1);
  assert.equal(isHeadFresh(head(-30), limits), false);
  assert.equal(isHeadFresh(head(5), limits), true);
});

test('custom limits and zero tolerance are honored without implicit defaults', (t) => {
  t.mock.method(Date, 'now', () => now);
  assert.equal(
    isHeadFresh(head(-180), {
      maxHeadAgeSeconds: 300,
      maxHeadTimeAheadSeconds: 0,
    }),
    true,
  );
  assert.equal(
    isHeadFresh(head(1), {
      maxHeadAgeSeconds: 300,
      maxHeadTimeAheadSeconds: 0,
    }),
    false,
  );
  assert.equal(
    isHeadFresh(head(), {
      maxHeadAgeSeconds: 0,
      maxHeadTimeAheadSeconds: 0,
    }),
    true,
  );
});

for (const number of [0, -1, 1.5, NaN, Infinity, 4294967296]) {
  test(`invalid head number ${number} cannot be fresh`, (t) => {
    t.mock.method(Date, 'now', () => now);
    assert.equal(
      isHeadFresh(
        {
          ...head(),
          head_block_number: number,
        },
        limits,
      ),
      false,
    );
  });
}

for (const field of ['maxHeadAgeSeconds', 'maxHeadTimeAheadSeconds']) {
  for (const value of [-1, 0.5, NaN, Infinity]) {
    test(`invalid ${field}=${value} is a configuration error`, () => {
      assert.throws(
        () =>
          isHeadFresh(head(), {
            ...limits,
            [field]: value,
          }),
        RangeError,
      );
    });
  }
}

test('malformed protocol time is rejected instead of accepted as fresh', () => {
  assert.throws(() =>
    isHeadFresh(
      {
        ...head(),
        time: '2026-02-30T12:00:00',
      },
      limits,
    ),
  );
});

test('equal head position compares byte values across independent buffers', () => {
  const before = head();
  const after = head();
  after.time = '2020-01-01T00:00:00';
  assert.notEqual(before.head_block_id, after.head_block_id);
  assert.equal(isSameHead(before, after), true);
});

test('a same-height reorganization has a different head', () => {
  const before = head();
  const after = head();
  after.head_block_id[19] = 2;
  assert.equal(isSameHead(before, after), false);
});

for (const number of [99, 101]) {
  test(`a move to block ${number} differs even if the node repeats the ID`, () => {
    const before = head();
    const after = head();
    after.head_block_number = number;
    assert.equal(isSameHead(before, after), false);
  });
}

test('head IDs of different lengths are not equal', () => {
  const before = head();
  const after = head();
  after.head_block_id = after.head_block_id.slice(0, 19);
  assert.equal(isSameHead(before, after), false);
});
