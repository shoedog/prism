import { Body, Controller, Get, Query as Input } from '@nestjs/common';
import { readdir as list } from 'node:fs';
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
@Controller()
class Routes {
  @Get()
  view(@Input() value: string) {
    list(value); // @SINK
  }
}
