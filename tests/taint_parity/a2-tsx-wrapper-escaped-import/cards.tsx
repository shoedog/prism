import { memo as m\u0065mo } from 'react';
export const View = m\u0065mo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
