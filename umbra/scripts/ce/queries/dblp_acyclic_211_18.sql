select count(*) from dblp8, dblp25, dblp1, dblp24, dblp22, dblp16, dblp3 where dblp8.s = dblp25.s and dblp25.d = dblp1.d and dblp1.s = dblp24.s and dblp24.d = dblp22.d and dblp22.s = dblp16.s and dblp16.d = dblp3.s;