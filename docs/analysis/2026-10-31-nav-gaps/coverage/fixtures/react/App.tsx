import { useState, useEffect, useContext, createContext } from 'react';
import { Html } from './Html';
import { Html2 } from './Html2';
const Ctx = createContext('');
export function App({ query }: { query: string }) {
  const [text, setText] = useState('');
  useEffect(() => {
    fetch('/api?q=' + query).then((r) => r.text()).then(setText);
  }, [query]);
  return (
    <Ctx.Provider value={text}>
      <Html html={text} />
      <Html2 html={text} />
      <Child />
    </Ctx.Provider>
  );
}
function Child() {
  const v = useContext(Ctx);
  return <a href={v}>link</a>;
}
