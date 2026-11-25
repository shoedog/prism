import { Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';
import { ReportsModule } from './reports.module';

@Module({ imports: [ReportsModule] })
export class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
