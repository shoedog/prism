import { memo } from 'react';
function entry() {
const View = memo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
  const t = acquire(); // @SRC
  return <View value={t}/>;
}
