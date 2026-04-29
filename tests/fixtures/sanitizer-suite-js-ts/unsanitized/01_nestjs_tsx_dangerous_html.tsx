import { Body, Controller, Post } from "@nestjs/common";

@Controller("pages")
export class PageController {
  @Post()
  create(@Body() body: CreateDto) {
    const html = body.htmlContent;
    return <div dangerouslySetInnerHTML={{ __html: html }} />;
  }
}

