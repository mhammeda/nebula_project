<template>
    <div class="homePage">
        <v-container fluid>
            <v-row dense>
                <v-col :cols="12">
                    <v-card elevation="0" color="primary">
                        <v-list-item two-line>
                            <v-list-item-content>
                                <v-list-item>
                                    <img
                                        class="text-left"
                                        src="../assets/logo.png"
                                        height="200px"
                                    />
                                </v-list-item>
                            </v-list-item-content>
                        </v-list-item>
                    </v-card>
                </v-col>
                <v-col :cols="12"> </v-col>
                <v-col :cols="9">
                    <AllPosts />
                </v-col>
                <v-col :cols="3">
                    <div class="userCards">
                        <CommunitiesArea />
                        <InboxDropdown />
                    </div>
                </v-col>
            </v-row>
        </v-container>
    </div>
</template>

<script>
// components.
import AllPosts from './posts/allPosts.vue'
import CommunitiesArea from './communities/CommunitiesArea.vue'
import InboxDropdown from './chat/InboxDropdown.vue'

export default {
    data() {
        return {
            url: '/internal/messages/unread',
        }
    },
    components: {
        AllPosts,
        CommunitiesArea,
        InboxDropdown,
    },
}
</script>

<style scoped>
h1,
h2 {
    font-weight: normal;
}
ul {
    list-style-type: none;
    padding: 0;
}
li {
    display: inline-block;
    margin: 0 10px;
}

.homePage {
    width: 100%;
    margin-bottom: 10px;
}
.homePage h1 {
    float: left;
    margin-left: 6%;
    font-size: 50px;
}
.homePage button {
    float: left;
}

.allPostsArea {
    float: left;
    width: 80%;
}

.userCards {
    float: right;
    margin-right: 20px;
    display: block;
    position: fixed;
}

.communitiesArea {
    margin-bottom: 20px;
}
</style>
