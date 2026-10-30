import { forwardRef } from 'react';
export const Island = forwardRef<HTMLDivElement, any>((props, ref) => {
  return <div ref={ref}/>;
});
