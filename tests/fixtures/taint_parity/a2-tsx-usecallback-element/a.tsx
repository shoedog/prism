import { useCallback } from 'react';
function Entry() {
  const View = useCallback((p: {value: string}) => {
  observe(p.value); // @SINK
  return null;
}, []);
  const t = acquire(); // @SRC
  return <View value={t}/>;
}
