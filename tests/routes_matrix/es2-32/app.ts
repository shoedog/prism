import { Controller, Get, Module, Post } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('stock')
class StockController {
  @Get('list')
  @Post('refresh')
  sync() {
    return [];
  }
}

@Module({ controllers: [StockController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
