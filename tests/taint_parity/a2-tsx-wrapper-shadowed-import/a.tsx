import { memo } from 'react';
function keep<F>(fn: F): F { return fn; }
function Entry(memo = keep) {
  const View = memo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
  return [0].map(() => {
    const t = acquire(); // @SRC
    return <View value={t}/>;
  });
}
