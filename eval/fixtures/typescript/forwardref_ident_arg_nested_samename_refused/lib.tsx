import { forwardRef } from 'react';
function IslandInner(props: any, ref: any) {
  return <div ref={ref}/>;
}
function helper() {
  function Island() { return 1; }
  return Island();
}
export const Island = forwardRef(IslandInner);
