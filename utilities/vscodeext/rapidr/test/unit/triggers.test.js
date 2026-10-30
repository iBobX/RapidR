// When typing opens the completion list by itself (src/triggers.js).
'use strict';

const assert = require('assert');
const { suggestsTypes } = require('../../src/triggers');

describe('suggestsTypes', () => {
    it('is true after the word AS, in any case', () => {
        for (const line of ['DIM test AS ', 'dim test as ', 'SUB Go(n AS ', 'CREATE Form AS ', 'FUNCTION F() AS ', '    DIM x, y AS ', 'AS ']) {
            assert.strictEqual(suggestsTypes(line), true, line);
        }
    });

    it('is false after other words, other characters, and a word that only ends in AS', () => {
        for (const line of ['DIM test ', 'PRINT "x" ', 'ALIAS ', 'x = CLASS ', 'has ', 'DIM a$ ', 'x.AS ', 'FOR i = 1 TO 3 ']) {
            assert.strictEqual(suggestsTypes(line), false, line);
        }
    });

    it('is false in a string or a comment', () => {
        assert.strictEqual(suggestsTypes('PRINT "dim x AS '), false);
        assert.strictEqual(suggestsTypes("' dim x AS "), false);
        assert.strictEqual(suggestsTypes('REM dim x AS '), false);
        assert.strictEqual(suggestsTypes('PRINT "a" \' b AS '), false);
        assert.strictEqual(suggestsTypes('PRINT "it\'s" : DIM x AS '), true);
    });
});
