select count(*) from dblp16, dblp7, dblp8, dblp2, dblp17, dblp1, dblp26, dblp21 where dblp16.s = dblp7.s and dblp7.s = dblp8.s and dblp8.s = dblp2.s and dblp2.s = dblp17.s and dblp17.s = dblp1.s and dblp1.d = dblp26.s and dblp26.d = dblp21.s;