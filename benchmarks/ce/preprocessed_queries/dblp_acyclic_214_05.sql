select count(*) from dblp8, dblp14, dblp21, dblp2, dblp16, dblp19, dblp17, dblp1 where dblp8.s = dblp14.s and dblp14.s = dblp21.s and dblp21.s = dblp2.s and dblp2.s = dblp16.s and dblp16.s = dblp19.s and dblp19.d = dblp17.s and dblp17.d = dblp1.s;