function generic_loop()
  x = source()
  for key, x in iter(x) do
    sink(x)
  end
  sink(x)
end

function numeric_loop()
  x = source()
  for x =
    start(x),
    limit(x),
    step(x)
  do
    sink(x)
  end
  sink(x)
end
