import React from 'react';
export function Button() {
  const click = (event: React.MouseEvent<HTMLButtonElement>) => {
    console.log(event.clientX);
  };
  return <button onClick={click}>go</button>;
}
