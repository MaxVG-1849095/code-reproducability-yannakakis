select count(*) from dblp23, dblp24, dblp22, dblp16, dblp1, dblp21 where dblp23.s = dblp24.s and dblp24.d = dblp22.d and dblp22.s = dblp16.s and dblp16.d = dblp1.s and dblp1.d = dblp21.s;