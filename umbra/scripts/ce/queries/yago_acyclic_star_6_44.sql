select count(*) from yago26, yago11, yago52, yago6 yago6_3, yago6 yago6_4, yago56 where yago26.s = yago11.s and yago11.s = yago52.s and yago52.s = yago6_3.s and yago6_3.s = yago6_4.s and yago6_4.s = yago56.s;