select count(*) from dblp1, dblp22, dblp24, dblp9, dblp12, dblp20 where dblp1.s = dblp22.s and dblp22.d = dblp24.d and dblp24.s = dblp9.s and dblp9.d = dblp12.s and dblp12.d = dblp20.s;