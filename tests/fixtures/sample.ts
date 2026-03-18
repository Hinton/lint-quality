import { something } from 'somewhere';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const x: any = {};

// eslint-disable-next-line no-unused-vars, no-undef
const y = 1;

const z = foo(); // eslint-disable-line no-console

// @ts-ignore
const a = bar();

// @ts-expect-error legacy code
const b = baz();

export { x, y, z, a, b };
