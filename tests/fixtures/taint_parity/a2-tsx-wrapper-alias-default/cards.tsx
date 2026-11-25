import { memo } from 'react';
const defaults = { value: "" };
export const View = memo(({value} = defaults) => {
  observe(value); // @SINK
  return null;
});
