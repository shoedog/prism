import { memo } from 'react';
export let View = memo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
