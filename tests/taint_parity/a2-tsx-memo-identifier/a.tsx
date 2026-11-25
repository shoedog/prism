import { memo } from 'react';
const Inner = (p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
};
const View = memo(Inner);
function entry() {
  const t = acquire(); // @SRC
  return <View value={t}/>;
}
