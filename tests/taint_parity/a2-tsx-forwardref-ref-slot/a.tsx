import { View } from './cards';
function entry() {
  const t = acquire(); // @SRC
  return <View value={t}/>;
}
