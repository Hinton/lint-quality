/* eslint-disable */

const foo = require('bar');

// eslint-disable-next-line
const x = eval('1+1');

/* eslint-disable no-eval, no-console */
console.log(eval('hello'));
/* eslint-enable */

module.exports = { foo, x };
