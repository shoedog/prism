import { Controller, Get, Module, Version } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('collections')
class CollectionsController {
  @Version('2')
  @Get('featured')
  featured() {
    return { items: [] };
  }

  @Get('latest')
  latest() {
    return { items: [] };
  }
}

@Module({ controllers: [CollectionsController] })
class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
