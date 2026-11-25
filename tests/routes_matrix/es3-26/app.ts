import { Controller, Get, Module, Redirect, Sse } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';
import { of } from 'rxjs';

@Controller('stock')
class StockController {
  @Sse('events')
  stream() { return of({ data: 'in stock' }); }

  @Get('latest')
  @Redirect('/stock/events', 302)
  latest() {}
}

@Module({ controllers: [StockController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
