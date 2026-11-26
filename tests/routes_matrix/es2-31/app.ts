import { Controller, Get, Module, VersioningType } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('stock')
class StockController {
  @Get()
  current() {
    return [];
  }
}

@Module({ controllers: [StockController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  app.enableVersioning({ type: VersioningType.URI, prefix: false, defaultVersion: '2' });
  await app.init();
  return app;
}
