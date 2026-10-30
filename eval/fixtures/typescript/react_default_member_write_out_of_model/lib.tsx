import React from 'react';
React.forwardRef = function fake(): any {
  return function Replacement() { return null; };
} as any;
export const Island = React.forwardRef((props: any, ref: any) => {
  return <div ref={ref}/>;
});
