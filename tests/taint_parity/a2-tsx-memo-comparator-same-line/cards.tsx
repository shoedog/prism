import { memo } from 'react';
export const View = memo((p: {value: string}) => (observe(p.value), null), (prev, next) => prev.value === next.value); // @SINK
