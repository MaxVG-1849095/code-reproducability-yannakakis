select count(*) from dblp7, dblp22, dblp4, dblp19, dblp5, dblp24 where dblp7.s = dblp22.s and dblp22.s = dblp4.s and dblp4.s = dblp19.s and dblp19.s = dblp5.s and dblp5.s = dblp24.s;