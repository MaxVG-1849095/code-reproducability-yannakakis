select count(*) from dblp22, dblp21, dblp26, dblp2, dblp19, dblp25 where dblp22.s = dblp21.s and dblp21.s = dblp26.s and dblp26.d = dblp2.s and dblp2.d = dblp19.s and dblp19.d = dblp25.s;