select count(*) from imdb2, imdb120, imdb100, imdb24, imdb5 where imdb2.d = imdb120.d and imdb120.d = imdb100.d and imdb100.d = imdb24.s and imdb24.s = imdb5.s;