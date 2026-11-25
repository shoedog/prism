import { Controller, Get, Headers, Res } from '@nestjs/common';
import type { Response } from 'express';
@Controller('editions')
export class Editions {
  @Get()
  show(@Res({ passthrough: true }) out: Response, @Headers('x-edition') inbound: string) {
    out.setHeader('X-Edition', 'preview');
    const outgoing = out.getHeader('X-Edition');
    return [outgoing, inbound];
  }
}
