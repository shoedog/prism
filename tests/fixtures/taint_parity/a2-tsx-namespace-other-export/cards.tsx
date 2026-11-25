import { memo } from 'react';
const View = memo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
function Other(p: {value: string}) { return null; }
export { Other as View };
