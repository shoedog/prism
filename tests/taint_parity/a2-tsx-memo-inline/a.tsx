import { memo } from 'react';
const View = memo((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
});
function entry() {
  const t = acquire(); // @SRC
  return <View value={t}/>;
}
