import { Body, Controller, Get, Query as Input } from '@nestjs/common';
import { readdir as list } from 'node:fs';
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
@Controller()
class Routes {
  @Get()
  other() { return 0; }
  view(@Body() value: string) {
    list(value); // @SINK
  }
}
