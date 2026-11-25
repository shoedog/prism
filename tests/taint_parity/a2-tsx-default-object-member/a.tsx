import Cards from './cards';
function entry() {
  const t = acquire(); // @SRC
  return <Cards.View value={t}/>;
}
