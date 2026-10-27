function do_block()
  x = source()
  observe(x)
  do
    x = clean()
  end
  sink(x)
end

function repeat_block()
  x = source()
  observe(x)
  repeat
    x = clean()
  until done()
  sink(x)
end
