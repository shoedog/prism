import { Controller, Module, Post, Req, type RawBodyRequest } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';
import type { Request } from 'express';

@Controller('preview')
class PreviewController {
  @Post()
  show(@Req() incoming: RawBodyRequest<Request>) {
    const bytes = incoming.rawBody;
    return bytes;
  }
}

@Module({ controllers: [PreviewController] })
class PreviewModule {}

export function createApp() {
  return NestFactory.create(PreviewModule, { rawBody: true });
}
