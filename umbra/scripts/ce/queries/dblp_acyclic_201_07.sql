select count(*) from dblp23, dblp5, dblp22, dblp8, dblp24, dblp3 where dblp23.s = dblp5.s and dblp5.s = dblp22.s and dblp22.s = dblp8.s and dblp8.s = dblp24.s and dblp24.s = dblp3.s;