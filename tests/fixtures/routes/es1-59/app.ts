import { Controller, Get, Version, VersioningType, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';

@Controller('reports')
class ReportsController {
  @Version('1')
  @Get('summary')
  summaryOne() {
    return ['one'];
  }

  @Version('2')
  @Get('summary')
  summaryTwo() {
    return ['two'];
  }
}

@Module({ controllers: [ReportsController] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  app.enableVersioning({ type: VersioningType.HEADER, header: 'X-Revision' });
  await app.init();
  return app;
}
