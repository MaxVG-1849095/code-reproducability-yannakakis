select count(*) from dblp24, dblp22, dblp12, dblp19, dblp21, dblp17, dblp8, dblp23 where dblp24.s = dblp22.s and dblp22.s = dblp12.s and dblp12.s = dblp19.s and dblp19.d = dblp21.s and dblp21.d = dblp17.s and dblp17.d = dblp8.s and dblp8.s = dblp23.s;