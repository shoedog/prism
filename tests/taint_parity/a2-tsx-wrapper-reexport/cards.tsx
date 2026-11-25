import { memo } from './bridge';
export const View = memo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
