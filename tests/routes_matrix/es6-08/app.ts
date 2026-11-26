import { Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';
import { LedgerModule } from './ledger.module';

@Module({ imports: [LedgerModule.register()] })
class AppModule {}

export async function createApp() {
  const app = await NestFactory.create(AppModule);
  await app.init();
  return app;
}
