select count(*) from imdb29, imdb1, imdb123, imdb2, imdb100, imdb38 where imdb29.s = imdb1.s and imdb1.s = imdb123.s and imdb123.d = imdb2.d and imdb2.d = imdb100.d and imdb100.d = imdb38.s;