import { Controller, Post, UploadedFiles, UseInterceptors } from '@nestjs/common';
import { FilesInterceptor } from '@nestjs/platform-express';
@Controller('documents')
export class Documents {
  @Post()
  @UseInterceptors(FilesInterceptor('files'))
  upload(@UploadedFiles() attachments: Array<{ originalname: string }>) {
    return attachments;
  }
}
